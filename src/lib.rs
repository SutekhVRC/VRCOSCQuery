use crate::http::json_models::{HostInfo, HostInfoExtensions};
use crate::http::OQHTTPHandler;
use crate::mdns::{get_target_service, OQMDNSHandler, OSC_JSON_SERVICE};
use mdns_sd::ServiceInfo;
use node::OSCQueryNode;
use std::net::{IpAddr, SocketAddrV4};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio::sync::watch;

#[cfg(not(test))]
use log::info;

#[cfg(test)]
use std::println as info;

pub mod http;
pub mod mdns;
pub mod node;

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
    vrchat_parameters: Option<OSCQueryNode>,
}

#[derive(Error, Debug)]
pub enum OQError {
    #[error("error from serde")]
    SerdeError(#[from] serde_json::Error),
    #[error("error from reqwest")]
    RwquestError(#[from] reqwest::Error),
    #[error("error from mdns")]
    MDNSError(#[from] mdns_sd::Error),
    #[error("no service daemon")]
    NoServiceDaemon,
    #[error("no mdns handler")]
    NoMdnsHandler,
}

impl OSCQuery {
    pub fn new(app_name: String, http_net: SocketAddrV4, osc_net: SocketAddrV4) -> Self {
        OSCQuery {
            app_name,
            http_net,
            osc_net,
            async_runtime: None,
            thread_tx: None,
            thread_rx: None,
            mdns_handler: None,
            vrchat_parameters: None,
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
        self.async_runtime
            .take()
            .unwrap()
            .shutdown_timeout(Duration::from_secs(10));
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

        let http_service =
            OQHTTPHandler::new(self.http_net, host_info, self.thread_rx.clone().unwrap());
        let http_thread = crate::http::start(http_service);
        self.async_runtime.as_mut().unwrap().spawn(http_thread);

        info!("[+] HTTP service running..");
    }

    pub fn register_mdns_service(&self) -> Result<(), OQError> {
        info!("[+] Registering mDNS service for {}", self.app_name);
        self.mdns_handler
            .as_ref()
            .ok_or_else(|| OQError::NoServiceDaemon)?
            .register()?;
        Ok(())
    }

    pub fn unregister_mdns_service(&self) {
        info!("[+] Unregistering mDNS service for {}", self.app_name);
    }

    pub fn mdns_search(
        &mut self,
        service_prefix: String,
        service_type: &'static str,
    ) -> Result<ServiceInfo, OQError> {
        info!("[+] Searching for {}", service_prefix);
        let s_info = get_target_service(
            self.mdns_handler
                .as_ref()
                .ok_or_else(|| OQError::NoServiceDaemon)?,
            service_prefix,
            service_type,
        )?;
        info!("[+] Got service info: {}", s_info.get_hostname());
        Ok(s_info)
    }

    pub fn populate_vrc_params(
        &mut self,
        service_prefix: String,
        service_type: &'static str,
    ) -> Result<(), OQError> {
        info!("[*] Populating parameters from TCP/JSON service..");

        let s_info = self.mdns_search(service_prefix, service_type)?;
        let host = s_info
            .get_addresses()
            .iter()
            .collect::<Vec<&IpAddr>>()
            .first()
            .take()
            .unwrap()
            .to_string();
        let index_enpdoint = format!("http://{}:{}/", host, s_info.get_port());

        info!("[*] Requesting index endpoint: {}", index_enpdoint);

        let http_res = reqwest::blocking::get(index_enpdoint)?;
        let json_res = http_res.text()?;

        let node_tree = match serde_json::from_str::<OSCQueryNode>(&json_res) {
            Ok(jr) => jr,
            Err(e) => {
                info!("[-] Failed to deserialize: {}\n{:?}", e, json_res);
                return Err(OQError::SerdeError(e));
            }
        };

        info!("[+] Successfully parsed index node tree.");
        self.vrchat_parameters = Some(node_tree);
        Ok(())
    }

    pub fn attempt_force_vrc_response_detect(&self, attempts: u64) -> Result<(), OQError> {
        let app_name = self.app_name.clone();
        let http_net = self.http_net.clone();
        std::thread::spawn(move || {
            for _ in 0..attempts {
                let mut mdns_force = OQMDNSHandler::new(app_name.clone(), http_net);
                mdns_force.start_daemon();
                mdns_force.register();
                get_target_service(&mdns_force, "VRChat-Client-".to_string(), OSC_JSON_SERVICE);
                mdns_force.unregister();
                mdns_force.shutdown_daemon();
            }
        });
        Ok(())
    }

    pub fn initialize_mdns(&mut self) -> Result<(), OQError> {
        self.mdns_handler = Some(OQMDNSHandler::new(self.app_name.clone(), self.http_net));
        self.mdns_handler
            .as_mut()
            .ok_or_else(|| OQError::NoMdnsHandler)?
            .start_daemon()?;
        Ok(())
    }

    pub fn shutdown_mdns(&mut self) -> Result<(), OQError> {
        let mut h = self
            .mdns_handler
            .take()
            .ok_or_else(|| OQError::NoMdnsHandler)?;
        h.shutdown_daemon()?;
        Ok(())
    }
}
