use std::net::Ipv4Addr;

use super::*;

#[test]
fn instantiate_30_seconds() {

    let osc_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9069);
    let http_addr = SocketAddrV4::new(Ipv4Addr::new(172, 19, 19, 244), 8080);

    let mut instance = OSCQuery::new("VibeCheck".to_string(), http_addr, osc_addr);
    instance.start_http_json();
    instance.register_mdns_service();

    loop {
    let _resolved_vrc_service = instance.mdns_search("VRChat-Client-".to_string());
    std::thread::sleep(Duration::from_secs(1));
    }
    //info!("[+] Got resolved VRC Service: {:?}", resolved_vrc_service);

    std::thread::sleep(Duration::from_secs(30));

    info!("[*] Stopping JSON service..");
    instance.stop_http_json();
    info!("[+] JSON service shutdown.");

    info!("[*] Stopping mDNS listener..");
    instance.unregister_mdns_service();
    info!("[+] mDNS listener shutdown.");
}