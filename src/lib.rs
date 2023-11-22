use std::net::SocketAddrV4;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[cfg(not(test))]
use log::info;
use mdns_sd::ServiceInfo;

#[cfg(test)]
use std::println as info;

use tokio::runtime::Runtime;
//use tokio
use tokio::sync::watch;

use crate::http::OQHTTPHandler;
use crate::http::host_info::{HostInfo, HostInfoExtensions};
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
    mdns_service: Option<OQMDNSHandler>,
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
            mdns_service: None,
        }
    }

    pub fn start_oq(&mut self) {

        
        let (thread_tx, thread_rx) = watch::channel::<AtomicBool>(AtomicBool::new(false));
        self.thread_tx = Some(thread_tx);
        self.thread_rx = Some(thread_rx);

        self.async_runtime = Some(Runtime::new().unwrap());
        info!("Started Async runtime..");

        self.start_http();
        self.start_mdns();
    }

    pub fn stop_oq(&mut self) {

        let tx = self.thread_tx.take().unwrap();
        tx.send(AtomicBool::new(false)).unwrap();
        
        info!("Sent shutdown signal to OSCQuery threads..");

        info!("Shutting down async runtime..");
        self.async_runtime.take().unwrap().shutdown_timeout(Duration::from_secs(10));
        info!("Async runtime successfully shutdown.");

    }

    fn start_http(&mut self) {

        info!("Staring HTTP service.. {}", self.http_net);

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

        info!("HTTP service running..");
    }

    fn start_mdns(&mut self) {

        info!("Starting mDNS service..");

        let mdns_handler = OQMDNSHandler::new(self.app_name.clone(), self.http_net);

        info!("Registering mDNS service for {}", self.app_name);
        mdns_handler.register();

        self.mdns_service = Some(mdns_handler);
    }

    pub fn mdns_search(&mut self, service_prefix: String) -> ServiceInfo {

        info!("Searching for {}", service_prefix);
        let s_info = get_target_service(self.mdns_service.take().unwrap(), service_prefix, OSC_JSON_SERVICE);
        info!("Got service info: {:?}", s_info);
        s_info
    }
}