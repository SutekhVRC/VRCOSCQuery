use crate::OQError;

use super::info;
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::SocketAddrV4;

pub const OSC_JSON_SERVICE: &'static str = "_oscjson._tcp.local.";
pub const OSC_SERVICE: &'static str = "_osc._udp.local.";

pub struct OQMDNSHandler {
    service_daemon: Option<ServiceDaemon>,
    service_info: ServiceInfo,
}

impl OQMDNSHandler {
    pub fn new(app_name: String, http_addr: SocketAddrV4) -> Result<Self, mdns_sd::Error> {
        let mdns_properties = vec![("mjau", "grr")];
        let ip_addr = *http_addr.ip();
        let port = http_addr.port();

        let service_info = ServiceInfo::new(
            "_oscjson._tcp.local.",
            app_name.as_str(),
            format!("{}.oscjson.tcp.local.", app_name).as_str(),
            ip_addr.to_string(),
            port,
            &mdns_properties[..],
        )?;

        Ok(OQMDNSHandler {
            service_daemon: None,
            service_info,
        })
    }

    pub fn start_daemon(&mut self) -> Result<(), OQError> {
        self.service_daemon = Some(ServiceDaemon::new()?);
        self.service_daemon
            .as_ref()
            .ok_or_else(|| OQError::NoServiceDaemon)?
            .disable_interface(IfKind::IPv6)?;
        Ok(())
    }

    pub fn shutdown_daemon(&mut self) -> Result<(), OQError> {
        self.service_daemon
            .take()
            .ok_or_else(|| OQError::NoServiceDaemon)?
            .shutdown()?;
        Ok(())
    }

    pub fn register(&self) -> Result<(), OQError> {
        self.service_daemon
            .as_ref()
            .ok_or_else(|| OQError::NoServiceDaemon)?
            .register(self.service_info.clone())?;
        info!("[+] Registered mDNS service.");
        Ok(())
    }
    pub fn unregister(&self) -> Result<(), OQError> {
        self.service_daemon
            .as_ref()
            .ok_or_else(|| OQError::NoServiceDaemon)?
            .unregister(&self.service_info.get_type())?;
        info!("[+] Unregistered {}", self.service_info.get_type());
        Ok(())
    }
}

pub fn get_target_service(
    mdns_handler: &OQMDNSHandler,
    match_prefix: String,
    s_type: &'static str,
) -> Result<ServiceInfo, OQError> {
    let service_receiver = mdns_handler
        .service_daemon
        .as_ref()
        .ok_or_else(|| OQError::NoServiceDaemon)?
        .browse(s_type)?;
    info!("[*] mDNS browsing..");

    loop {
        if let Some(event) = service_receiver.recv().ok() {
            match event {
                ServiceEvent::ServiceResolved(service_info) => {
                    if service_info.get_fullname().starts_with(&match_prefix) {
                        return Ok(service_info);
                    }
                }
                e => info!("[?] NOT RESOLVED {:?}", e),
            }
        }
    }
}
