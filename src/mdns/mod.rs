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
    pub fn new(app_name: String, http_addr: SocketAddrV4) -> Self {
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
        )
        .unwrap();

        OQMDNSHandler {
            service_daemon: None,
            service_info,
        }
    }

    pub fn start_daemon(&mut self) {
        self.service_daemon = Some(ServiceDaemon::new().unwrap());
        self.service_daemon
            .as_ref()
            .unwrap()
            .disable_interface(IfKind::IPv6)
            .unwrap();
    }

    pub fn shutdown_daemon(&mut self) {
        self.service_daemon.take().unwrap().shutdown().unwrap();
    }

    pub fn register(&self) {
        self.service_daemon
            .as_ref()
            .unwrap()
            .register(self.service_info.clone())
            .unwrap();
        info!("[+] Registered mDNS service.");
    }
    pub fn unregister(&self) {
        self.service_daemon
            .as_ref()
            .unwrap()
            .unregister(&self.service_info.get_type())
            .unwrap();
        info!("[+] Unregistered {}", self.service_info.get_type());
    }
}

pub fn get_target_service(
    mdns_handler: &OQMDNSHandler,
    match_prefix: String,
    s_type: &'static str,
) -> ServiceInfo {
    let service_receiver = mdns_handler
        .service_daemon
        .as_ref()
        .unwrap()
        .browse(s_type)
        .unwrap();
    info!("[*] mDNS browsing..");

    loop {
        let event = service_receiver.recv().unwrap();
        match event {
            ServiceEvent::ServiceResolved(service_info) => {
                if service_info.get_fullname().starts_with(&match_prefix) {
                    return service_info;
                }
            }
            e => info!("[?] NOT RESOLVED {:?}", e),
        }
    }
}
