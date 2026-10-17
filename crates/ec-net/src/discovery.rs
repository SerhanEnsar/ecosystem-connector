//! Finding devices on the local network with mDNS / DNS-SD.
//!
//! The announcement contains the device id and name. It is not authenticated, and does
//! not need to be: connecting still requires the TLS handshake with the pinned
//! certificate, so a fake announcement can at most point us to an address where the
//! handshake fails.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ec_core::{DeviceId, DeviceIdentity, sanitize_device_name};
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::{Error, Result};

/// The DNS-SD service type devices announce.
pub const SERVICE_TYPE: &str = "_ecconnect._udp.local.";
const PROTOCOL_VERSION: &str = "1";
/// Most addresses kept per discovered device.
const MAX_ADDRS: usize = 8;

/// A device seen on the network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDevice {
    /// The id it announced.
    pub id: DeviceId,
    /// The name it announced (sanitized).
    pub name: String,
    /// Where its node listens.
    pub addrs: Vec<SocketAddr>,
}

/// Announces this device on the local network while it is alive.
pub struct Advertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Advertiser {
    /// Starts announcing `identity` with the node's `port`.
    pub fn start(identity: &DeviceIdentity, port: u16) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let fullname = register(&daemon, identity, port)?;
        Ok(Self { daemon, fullname })
    }
}

impl Drop for Advertiser {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

type DeviceMap = BTreeMap<DeviceId, (DiscoveredDevice, String)>;

/// Announces this device and keeps a live view of the other devices on the network.
///
/// Unlike [`browse`], which looks for a fixed time, this runs until dropped and calls
/// `on_change` whenever a device appears, changes address or leaves.
pub struct Presence {
    daemon: ServiceDaemon,
    fullname: String,
    devices: Arc<Mutex<DeviceMap>>,
}

impl Presence {
    /// Starts announcing `identity` on `port` and watching for other devices.
    pub fn start(
        identity: &DeviceIdentity,
        port: u16,
        on_change: impl Fn() + Send + 'static,
    ) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let fullname = register(&daemon, identity, port)?;
        let events = daemon.browse(SERVICE_TYPE)?;
        let devices: Arc<Mutex<DeviceMap>> = Arc::default();
        let me = identity.id();

        let shared = devices.clone();
        std::thread::Builder::new()
            .name("ec-presence".into())
            .spawn(move || {
                // Ends when the daemon shuts down and the channel closes.
                while let Ok(event) = events.recv() {
                    let changed = match event {
                        ServiceEvent::ServiceResolved(service) => {
                            match parse_resolved(&service).filter(|d| d.id != me) {
                                Some(device) => {
                                    let entry = (device.clone(), service.fullname.clone());
                                    lock(&shared).insert(device.id, entry.clone()) != Some(entry)
                                }
                                None => false,
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            let mut map = lock(&shared);
                            let before = map.len();
                            map.retain(|_, (_, name)| *name != fullname);
                            map.len() != before
                        }
                        ServiceEvent::SearchStopped(_) => break,
                        _ => false,
                    };
                    if changed {
                        on_change();
                    }
                }
            })
            .map_err(Error::Io)?;
        Ok(Self {
            daemon,
            fullname,
            devices,
        })
    }

    /// Devices currently seen on the network (not this one).
    pub fn devices(&self) -> Vec<DiscoveredDevice> {
        lock(&self.devices)
            .values()
            .map(|(d, _)| d.clone())
            .collect()
    }

    /// Looks up one device.
    pub fn get(&self, id: &DeviceId) -> Option<DiscoveredDevice> {
        lock(&self.devices).get(id).map(|(d, _)| d.clone())
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
    }
}

fn lock(map: &Mutex<DeviceMap>) -> std::sync::MutexGuard<'_, DeviceMap> {
    map.lock().unwrap_or_else(|e| e.into_inner())
}

fn register(daemon: &ServiceDaemon, identity: &DeviceIdentity, port: u16) -> Result<String> {
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
    Ok(fullname)
}

/// Reads a device from a resolved DNS-SD record. Returns `None` for other protocol
/// versions, a missing or malformed id, or no usable address.
fn parse_resolved(service: &ResolvedService) -> Option<DiscoveredDevice> {
    if service.get_property_val_str("v") != Some(PROTOCOL_VERSION) {
        return None;
    }
    let id = service
        .get_property_val_str("id")?
        .parse::<DeviceId>()
        .ok()?;
    let name = sanitize_device_name(service.get_property_val_str("name").unwrap_or(""));
    let mut addrs: Vec<SocketAddr> = service
        .get_addresses()
        .iter()
        .map(|ip| SocketAddr::new(ip.to_ip_addr(), service.get_port()))
        .filter(SocketAddr::is_ipv4)
        .collect();
    addrs.sort();
    addrs.dedup();
    addrs.truncate(MAX_ADDRS);
    (!addrs.is_empty()).then_some(DiscoveredDevice { id, name, addrs })
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
        let Some(device) = parse_resolved(&service) else {
            continue;
        };
        let entry = found.entry(device.id).or_insert_with(|| DiscoveredDevice {
            addrs: Vec::new(),
            ..device.clone()
        });
        for addr in device.addrs {
            if !entry.addrs.contains(&addr) && entry.addrs.len() < MAX_ADDRS {
                entry.addrs.push(addr);
            }
        }
    }
    let _ = daemon.shutdown();
    Ok(found.into_values().collect())
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
