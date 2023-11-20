use std::net::SocketAddrV4;

use tokio::runtime::Runtime;

mod mdns;
mod http;

#[cfg(test)]
mod tests;


pub struct OSCQuery {
    http_net: SocketAddrV4,
    async_runtime: Option<Runtime>,
}

impl OSCQuery {
    pub fn new(http_net: SocketAddrV4) -> Self {
        OSCQuery {
            http_net,
            async_runtime: None,
        }
    }

    pub fn start_oq(&mut self) {
        
        self.async_runtime = Some(Runtime::new().unwrap());

        self.start_http();

    }

    fn start_http(&mut self) {

        
    }
}