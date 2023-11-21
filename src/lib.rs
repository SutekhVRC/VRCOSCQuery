use std::net::{SocketAddrV4, Ipv4Addr};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use log::info;
use tokio::runtime::Runtime;
//use tokio
use tokio::sync::watch;

use crate::http::OQHTTPHandler;
use crate::http::host_info::{HostInfo, HostInfoExtensions};

mod mdns;
mod http;

#[cfg(test)]
mod tests;


pub struct OSCQuery {
    http_net: SocketAddrV4,
    async_runtime: Option<Runtime>,
    thread_tx: Option<watch::Sender<AtomicBool>>,
    thread_rx: Option<watch::Receiver<AtomicBool>>,
}

impl OSCQuery {
    pub fn new(http_net: SocketAddrV4) -> Self {
        OSCQuery {
            http_net,
            async_runtime: None,
            thread_tx: None,
            thread_rx: None,
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

        info!("Staring HTTP service..");

        let extensions = HostInfoExtensions {
            access: true,
            clipmode: false,
            range: true,
            _type: true,
            value: true,
        };

        let host_info = HostInfo {
            name: "VRCOSCQuery Test",
            extensions,
            osc_ip: "127.0.0.1",
            osc_port: 8080,
            osc_transport: "UDP",
        };

        let http_service = OQHTTPHandler::new(self.http_net, host_info, self.thread_rx.clone().unwrap());
        let http_thread = crate::http::start(http_service);
        self.async_runtime.as_mut().unwrap().spawn(http_thread);

        info!("HTTP service running..");
    }

    fn start_mdns(&mut self) {


    }
}