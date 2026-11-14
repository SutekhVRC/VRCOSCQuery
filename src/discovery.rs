//! Finding OSCQuery services, such as VRChat's, and reading them.

use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use log::debug;
use mdns_sd::{ServiceDaemon, ServiceEvent};

use crate::http::HostInfo;
use crate::mdns::{OSC_JSON_SERVICE, VRCHAT_PREFIX};
use crate::node::OSCQueryNode;
use crate::OQError;

/// A service found over mDNS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Its full mDNS name, e.g. `VRChat-Client-ABC123._oscjson._tcp.local.`.
    pub name: String,
    /// Where to reach it. For an `_oscjson._tcp` service, its HTTP server.
    pub addr: SocketAddr,
}

impl Endpoint {
    /// An HTTP URL for `path` (e.g. `/?HOST_INFO`) on this endpoint.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

/// Looks for a service of `service_type` (e.g. [`OSC_JSON_SERVICE`]) whose
/// name starts with `name_prefix`, for up to `timeout`. `Ok(None)` if none
/// turned up in time.
pub async fn find_service(
    service_type: &str,
    name_prefix: &str,
    timeout: Duration,
) -> Result<Option<Endpoint>, OQError> {
    let service_type = service_type.to_owned();
    let name_prefix = name_prefix.to_owned();
    tokio::task::spawn_blocking(move || find_blocking(&service_type, &name_prefix, timeout))
        .await
        .map_err(|_| OQError::TaskFailed)?
}

/// Looks for VRChat's OSCQuery service (its HTTP server) for up to
/// `timeout`. `Ok(None)` if VRChat isn't running or wasn't found in time.
pub async fn find_vrchat(timeout: Duration) -> Result<Option<Endpoint>, OQError> {
    find_service(OSC_JSON_SERVICE, VRCHAT_PREFIX, timeout).await
}

/// A service's `/?HOST_INFO`, e.g. the address VRChat receives OSC on.
pub async fn host_info(endpoint: &Endpoint, timeout: Duration) -> Result<HostInfo, OQError> {
    Ok(serde_json::from_str(
        &get(&endpoint.url("/?HOST_INFO"), timeout).await?,
    )?)
}

/// The node at `path` in a service's tree, with everything under it. For
/// VRChat, `/avatar/parameters` gives every avatar parameter with its
/// current value.
pub async fn node(
    endpoint: &Endpoint,
    path: &str,
    timeout: Duration,
) -> Result<OSCQueryNode, OQError> {
    Ok(serde_json::from_str(
        &get(&endpoint.url(path), timeout).await?,
    )?)
}

async fn get(url: &str, timeout: Duration) -> Result<String, OQError> {
    Ok(reqwest::Client::builder()
        .timeout(timeout)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

fn find_blocking(
    service_type: &str,
    name_prefix: &str,
    timeout: Duration,
) -> Result<Option<Endpoint>, OQError> {
    let daemon = ServiceDaemon::new()?;
    let events = daemon.browse(service_type)?;
    let deadline = Instant::now() + timeout;
    let found = loop {
        match events.recv_deadline(deadline) {
            Ok(ServiceEvent::ServiceResolved(service))
                if service.get_fullname().starts_with(name_prefix) =>
            {
                let Some(ip) =
                    preferred_address(service.get_addresses().iter().map(|ip| ip.to_ip_addr()))
                else {
                    continue;
                };
                break Some(Endpoint {
                    name: service.get_fullname().to_owned(),
                    addr: SocketAddr::new(ip, service.get_port()),
                });
            }
            Ok(_) => continue,
            Err(_) => {
                debug!(
                    "No {} service starting with {} found",
                    service_type, name_prefix
                );
                break None;
            }
        }
    };
    let _ = daemon.stop_browse(service_type);
    let _ = daemon.shutdown();
    Ok(found)
}

/// This machine's address if it's advertised (the service is local), then
/// any IPv4 address, then anything.
fn preferred_address(addresses: impl Iterator<Item = IpAddr> + Clone) -> Option<IpAddr> {
    addresses
        .clone()
        .find(IpAddr::is_loopback)
        .or_else(|| addresses.clone().find(IpAddr::is_ipv4))
        .or_else(|| addresses.clone().next())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn prefers_loopback_then_ipv4() {
        let lan = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5));
        let v6 = IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));
        let local = IpAddr::V4(Ipv4Addr::LOCALHOST);
        assert_eq!(preferred_address([v6, lan, local].into_iter()), Some(local));
        assert_eq!(preferred_address([v6, lan].into_iter()), Some(lan));
        assert_eq!(preferred_address([v6].into_iter()), Some(v6));
        assert_eq!(preferred_address([].into_iter()), None);
    }
}
