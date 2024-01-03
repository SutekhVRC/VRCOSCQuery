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

#[derive(Error, Debug)]
pub enum OQError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP request error: {0}")]
    RwquestError(#[from] reqwest::Error),
    #[error("DNS error: {0}")]
    MDNSError(#[from] mdns_sd::Error),
    #[error("Tokio receive error: {0}")]
    ReceiveError(#[from] tokio::sync::watch::error::RecvError),
    #[error("Tokio send error: {0}")]
    SendError(#[from] tokio::sync::watch::error::SendError<AtomicBool>),
    #[error("ThreadJoinerror")]
    ThreadJoin,
    #[error("No bound address on http handler")]
    NoBoundAddress,
    #[error("No tx thread")]
    NoTxThread,
    #[error("No runtime")]
    NoRuntime,
    #[error("No host ip")]
    NoHostIP,
    #[error("Invalid HTTP buffer")]
    InvalidHttpBuffer,
}

pub struct OSCQuery {
    app_name: String,
    http_net: SocketAddrV4,
    osc_net: SocketAddrV4,
    async_runtime: Option<Runtime>,
    thread_tx: Option<watch::Sender<AtomicBool>>,
    thread_rx: Option<watch::Receiver<AtomicBool>>,
    mdns_handler: OQMDNSHandler,
    vrchat_parameters: Option<OSCQueryNode>,
}

impl OSCQuery {
    pub fn new(app_name: String, http_net: SocketAddrV4, osc_net: SocketAddrV4) -> Self {
        let mdns_handler = OQMDNSHandler::new(app_name.clone(), http_net).expect("could not create mdns handler");
        OSCQuery {
            app_name,
            http_net,
            osc_net,
            async_runtime: None,
            thread_tx: None,
            thread_rx: None,
            mdns_handler,
            vrchat_parameters: None,
        }
    }

    pub fn start_http_json(&mut self) -> Result<(), OQError> {
        let (thread_tx, thread_rx) = watch::channel::<AtomicBool>(AtomicBool::new(false));
        self.thread_tx = Some(thread_tx);
        self.thread_rx = Some(thread_rx);

        self.async_runtime = Some(Runtime::new()?);
        info!("[+] Started Async runtime.");
        self.start_http()?;
        info!("[+] Started HTTP server.");
        Ok(())
    }

    pub fn stop_http_json(&mut self) -> Result<(), OQError> {
        let tx = self.thread_tx.take().ok_or_else(|| OQError::NoTxThread)?;
        tx.send(AtomicBool::new(false))?;

        info!("[+] Sent shutdown signal to OSCQuery threads..");

        info!("[+] Shutting down async runtime..");
        self.async_runtime
            .take()
            .ok_or_else(|| OQError::NoRuntime)?
            .shutdown_timeout(Duration::from_secs(10));
        info!("[+] Async runtime successfully shutdown.");
        Ok(())
    }

    fn start_http(&mut self) -> Result<(), OQError> {
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

        let http_service = OQHTTPHandler::new(
            self.http_net,
            host_info,
            self.thread_rx.clone().ok_or_else(|| OQError::NoTxThread)?,
        );
        let http_thread = crate::http::start(http_service);
        self.async_runtime
            .as_mut()
            .ok_or_else(|| OQError::NoRuntime)?
            .spawn(http_thread);

        info!("[+] HTTP service running..");
        Ok(())
    }

    pub fn register_mdns_service(&self) -> Result<(), OQError> {
        info!("[+] Registering mDNS service for {}", self.app_name);
        self.mdns_handler.register()?;
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
        let s_info = get_target_service(&self.mdns_handler, service_prefix, service_type)?;
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
            .ok_or_else(|| OQError::NoHostIP)?
            .to_string();
        let index_enpdoint = format!("http://{}:{}/", host, s_info.get_port());

        info!("[*] Requesting index endpoint: {}", index_enpdoint);

        let http_res = reqwest::blocking::get(index_enpdoint)?;
        let json_res = http_res.text()?;
        let node_tree = serde_json::from_str::<OSCQueryNode>(&json_res)?;

        info!("[+] Successfully parsed index node tree.");
        self.vrchat_parameters = Some(node_tree);
        Ok(())
    }

    pub fn attempt_force_vrc_response_detect(&self, attempts: u64) -> Result<(), OQError> {
        let app_name = self.app_name.clone();
        let http_net = self.http_net.clone();
        let thread = std::thread::spawn(move || -> Result<(), OQError> {
            for _ in 0..attempts {
                if let Some(mut mdns_force) = OQMDNSHandler::new(app_name.clone(), http_net).ok() {
                    mdns_force.register()?;
                    get_target_service(
                        &mdns_force,
                        "VRChat-Client-".to_string(),
                        OSC_JSON_SERVICE,
                    )?;
                    mdns_force.unregister()?;
                    mdns_force.shutdown_daemon()?;
                }
            }
            Ok(())
        });
        thread.join().expect("Thread panic")?;
        Ok(())
    }

    pub fn shutdown_mdns(&mut self) -> Result<(), OQError> {
        self.mdns_handler.shutdown_daemon()?;
        Ok(())
    }
}
