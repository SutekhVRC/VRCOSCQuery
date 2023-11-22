use std::net::Ipv4Addr;

use super::*;

#[test]
fn instantiate_30_seconds() {

    let osc_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9079);
    let http_addr = SocketAddrV4::new(Ipv4Addr::new(172, 19, 19, 244), 8080);

    let mut instance = OSCQuery::new("BLOOP".to_string(), http_addr, osc_addr);
    instance.start_oq();
    instance.mdns_search("VRChat-Client-".to_string());
    std::thread::sleep(Duration::from_secs(30));
    info!("Sending stop signal!");
    instance.stop_oq();
}