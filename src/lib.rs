use std::net::SocketAddrV4;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[cfg(not(test))]
use log::info;

#[cfg(test)]
use std::println as info;

use mdns_sd::ServiceInfo;

use tokio::runtime::Runtime;
use tokio::sync::watch;

use crate::http::OQHTTPHandler;
use crate::http::json_models::{HostInfo, HostInfoExtensions};
use crate::mdns::{OQMDNSHandler, get_target_service, OSC_JSON_SERVICE};

pub mod mdns;
pub mod http;

#[cfg(test)]
mod tests;


pub struct OSCQuery {
    app_name: String,
    http_net: SocketAddrV4,
    osc_net: SocketAddrV4,
    async_runtime: Option<Runtime>,
    thread_tx: Option<watch::Sender<AtomicBool>>,
    thread_rx: Option<watch::Receiver<AtomicBool>>,
    mdns_handler: Option<OQMDNSHandler>,
}

impl OSCQuery {
    pub fn new(app_name: String, http_net: SocketAddrV4, osc_net: SocketAddrV4) -> Self {

        let mdns_handler = Some(OQMDNSHandler::new(app_name.clone(), http_net));
        
        OSCQuery {
            app_name,
            http_net,
            osc_net,
            async_runtime: None,
            thread_tx: None,
            thread_rx: None,
            mdns_handler,
        }
    }

    pub fn start_http_json(&mut self) {

        let (thread_tx, thread_rx) = watch::channel::<AtomicBool>(AtomicBool::new(false));
        self.thread_tx = Some(thread_tx);
        self.thread_rx = Some(thread_rx);

        self.async_runtime = Some(Runtime::new().unwrap());
        info!("[+] Started Async runtime.");
        self.start_http();
        info!("[+] Started HTTP server.");
    }

    pub fn stop_http_json(&mut self) {

        let tx = self.thread_tx.take().unwrap();
        tx.send(AtomicBool::new(false)).unwrap();
        
        info!("[+] Sent shutdown signal to OSCQuery threads..");

        info!("[+] Shutting down async runtime..");
        self.async_runtime.take().unwrap().shutdown_timeout(Duration::from_secs(10));
        info!("[+] Async runtime successfully shutdown.");

    }

    fn start_http(&mut self) {

        info!("[+] Staring HTTP service.. {}", self.http_net);

        let extensions = HostInfoExtensions {
            access: true,
            clipmode: false,
            range: true,
            _type: true,
            value: true,
        };

        let host_info = HostInfo {
            name: self.app_name.clone(),
            extensions,
            osc_ip: self.osc_net.ip().to_string(),
            osc_port: self.osc_net.port(),
            osc_transport: "UDP",
        };

        let http_service = OQHTTPHandler::new(self.http_net, host_info, self.thread_rx.clone().unwrap());
        let http_thread = crate::http::start(http_service);
        self.async_runtime.as_mut().unwrap().spawn(http_thread);

        info!("[+] HTTP service running..");
    }

    pub fn register_mdns_service(&self) {
        info!("[+] Registering mDNS service for {}", self.app_name);
        self.mdns_handler.as_ref().unwrap().register();
    }

    pub fn unregister_mdns_service(&self) {
        info!("[+] Unregistering mDNS service for {}", self.app_name);
    }

    pub fn mdns_search(&mut self, service_prefix: String, service_type: &'static str) -> ServiceInfo {

        info!("[+] Searching for {}", service_prefix);
        let s_info = get_target_service(self.mdns_handler.as_ref().unwrap(), service_prefix, service_type);
        info!("[+] Got service info: {}", s_info.get_hostname());
        s_info
    }

    pub fn attempt_force_vrc_response_detect(&self) {
        get_target_service(self.mdns_handler.as_ref().unwrap(), "VRChat-Client-".to_string(), OSC_JSON_SERVICE);
    }
}