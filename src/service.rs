//! Advertising an app's OSC port over OSCQuery.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use log::{debug, info, warn};
use mdns_sd::{IfKind, ServiceDaemon, ServiceInfo};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::http::{self, HostInfo, HostInfoExtensions, Responses};
use crate::mdns::{dns_label, OSC_JSON_SERVICE, OSC_SERVICE};
use crate::OQError;

/// How long [`OscQueryService::stop`] waits for each goodbye announcement.
const UNREGISTER_WAIT: Duration = Duration::from_millis(250);

/// An app's OSCQuery service: advertises the app over mDNS and serves the
/// OSCQuery HTTP endpoints, so VRChat sends OSC to the app's port.
///
/// The services are advertised on this machine's network interfaces (mDNS
/// doesn't use loopback), and the HTTP server listens on all of them, so
/// VRChat can reach it at whichever address it sees. `HOST_INFO` still
/// gives the OSC address the app chose, e.g. 127.0.0.1.
///
/// It advertises two services, both named after the app:
/// - `_oscjson._tcp`: the HTTP server, which VRChat queries. Its
///   `/?HOST_INFO` gives the OSC address, and its tree includes
///   `/avatar/change`, which is what tells VRChat to send avatar data.
/// - `_osc._udp`: the OSC port itself, for other OSC tools.
///
/// Stops advertising when [stopped](Self::stop) or dropped.
pub struct OscQueryService {
    osc_addr: SocketAddrV4,
    http_addr: SocketAddrV4,
    daemon: ServiceDaemon,
    services: Vec<ServiceInfo>,
    shutdown: watch::Sender<bool>,
    server: JoinHandle<()>,
    stopped: bool,
}

impl OscQueryService {
    /// Starts advertising `osc_addr`, the address the app receives OSC on,
    /// under `name`. The HTTP server gets a free port.
    ///
    /// If `osc_addr` is unspecified (0.0.0.0), `HOST_INFO` gives 127.0.0.1.
    /// Must be called from within a tokio runtime.
    pub async fn start(name: &str, osc_addr: SocketAddrV4) -> Result<Self, OQError> {
        let advertised_ip = if osc_addr.ip().is_unspecified() {
            Ipv4Addr::LOCALHOST
        } else {
            *osc_addr.ip()
        };
        let osc_addr = SocketAddrV4::new(advertised_ip, osc_addr.port());

        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;
        let http_addr = match listener.local_addr()? {
            SocketAddr::V4(addr) => addr,
            SocketAddr::V6(_) => unreachable!("bound to an IPv4 address"),
        };

        let host_info = HostInfo {
            name: name.to_owned(),
            extensions: HostInfoExtensions {
                access: true,
                clipmode: false,
                range: true,
                _type: true,
                value: true,
            },
            osc_ip: advertised_ip.to_string(),
            osc_port: osc_addr.port(),
            osc_transport: "UDP".to_owned(),
        };
        let responses = Arc::new(Responses::new(&host_info, app_tree())?);
        let (shutdown, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(http::serve(listener, responses, shutdown_rx));

        // From here on, stop the server if anything fails
        let advertise = || -> Result<(ServiceDaemon, Vec<ServiceInfo>), OQError> {
            let daemon = ServiceDaemon::new()?;
            if let Err(e) = daemon.disable_interface(IfKind::IPv6) {
                debug!("Couldn't disable IPv6 for mDNS: {}", e);
            }
            let label = dns_label(name);
            let host = format!("{label}.local.");
            let properties = [("txtvers", "1")];
            // No addresses: with addr_auto, mDNS fills in this machine's
            // interface addresses when registering
            let no_addresses: &[IpAddr] = &[];
            let services = vec![
                ServiceInfo::new(
                    OSC_JSON_SERVICE,
                    &label,
                    &host,
                    no_addresses,
                    http_addr.port(),
                    &properties[..],
                )?
                .enable_addr_auto(),
                ServiceInfo::new(
                    OSC_SERVICE,
                    &label,
                    &host,
                    no_addresses,
                    osc_addr.port(),
                    &properties[..],
                )?
                .enable_addr_auto(),
            ];
            for service in &services {
                daemon.register(service.clone())?;
            }
            Ok((daemon, services))
        };
        let (daemon, services) = match advertise() {
            Ok(advertised) => advertised,
            Err(e) => {
                let _ = shutdown.send(true);
                server.abort();
                return Err(e);
            }
        };

        info!(
            "OSCQuery: advertising {} (OSC on {}, HTTP on {})",
            name, osc_addr, http_addr
        );
        Ok(Self {
            osc_addr,
            http_addr,
            daemon,
            services,
            shutdown,
            server,
            stopped: false,
        })
    }

    /// The OSC address being advertised.
    pub fn osc_addr(&self) -> SocketAddrV4 {
        self.osc_addr
    }

    /// The OSCQuery HTTP server's address. It listens on every interface
    /// (0.0.0.0), so connect to it on 127.0.0.1 from this machine.
    pub fn http_addr(&self) -> SocketAddrV4 {
        self.http_addr
    }

    /// Announces the services again. Useful if VRChat started after the
    /// service and hasn't picked it up.
    pub fn reannounce(&self) -> Result<(), OQError> {
        for service in &self.services {
            self.daemon.register(service.clone())?;
        }
        Ok(())
    }

    /// Stops advertising: sends mDNS goodbyes so VRChat stops sending to
    /// the port, shuts down mDNS and stops the HTTP server, freeing its
    /// port. Dropping the service does the same.
    pub fn stop(mut self) {
        self.shut_down();
    }

    fn shut_down(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        for service in &self.services {
            match self.daemon.unregister(service.get_fullname()) {
                Ok(status) => {
                    let _ = status.recv_timeout(UNREGISTER_WAIT);
                }
                Err(e) => warn!(
                    "OSCQuery: couldn't unregister {}: {}",
                    service.get_fullname(),
                    e
                ),
            }
        }
        if let Err(e) = self.daemon.shutdown() {
            debug!("OSCQuery: mDNS shutdown failed: {}", e);
        }
        let _ = self.shutdown.send(true);
        self.server.abort();
        info!("OSCQuery: stopped advertising {}", self.osc_addr);
    }
}

impl Drop for OscQueryService {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// The node tree an app serves. `/avatar/change` is what VRChat looks for
/// before sending a service avatar data.
pub(crate) fn app_tree() -> Value {
    json!({
        "DESCRIPTION": "root node",
        "FULL_PATH": "/",
        "ACCESS": 0,
        "CONTENTS": {
            "avatar": {
                "FULL_PATH": "/avatar",
                "ACCESS": 0,
                "CONTENTS": {
                    "change": {
                        "DESCRIPTION": "Avatar ID, updated whenever the user switches into a valid avatar.",
                        "FULL_PATH": "/avatar/change",
                        "ACCESS": 2,
                        "TYPE": "s"
                    }
                }
            }
        }
    })
}
