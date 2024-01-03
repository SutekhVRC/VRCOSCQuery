use std::net::Ipv4Addr;

use super::*;

#[test]
#[ignore]
fn instantiate_10_seconds() {
    let osc_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9069);
    let http_addr = SocketAddrV4::new(Ipv4Addr::new(172, 19, 19, 244), 8080);

    let mut instance = OSCQuery::new("VibeCheck".to_string(), http_addr, osc_addr);
    instance.start_http_json().ok();
    instance.register_mdns_service().ok();

    let _resolved_vrc_service =
        instance.mdns_search("VRChat-Client-".to_string(), OSC_JSON_SERVICE);
    std::thread::sleep(Duration::from_secs(10));

    info!("[*] Stopping JSON service..");
    instance.stop_http_json().ok();
    info!("[+] JSON service shutdown.");

    info!("[*] Stopping mDNS listener..");
    instance.unregister_mdns_service();
    info!("[+] mDNS listener shutdown.");
}

#[test]
#[ignore]
fn test_node_parse() {
    let osc_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9069);
    let http_addr = SocketAddrV4::new(Ipv4Addr::new(172, 19, 19, 244), 8080);

    let mut instance = OSCQuery::new("VibeCheck".to_string(), http_addr, osc_addr);
    instance.start_http_json().ok();
    instance.register_mdns_service().ok();

    instance
        .populate_vrc_params("VRChat-Client-".to_owned(), OSC_JSON_SERVICE)
        .ok();
    std::thread::sleep(Duration::from_secs(10));

    info!("[*] Stopping JSON service..");
    instance.stop_http_json().ok();
    info!("[+] JSON service shutdown.");

    info!("[*] Stopping mDNS listener..");
    instance.unregister_mdns_service();
    instance.shutdown_mdns().ok();
    info!("[+] mDNS listener shutdown.");
}

#[test]
#[ignore]
fn test_vrchat_force_discover() {
    let osc_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9069);
    let http_addr = SocketAddrV4::new(Ipv4Addr::new(172, 19, 19, 244), 8080);

    let mut instance = OSCQuery::new("VibeCheck".to_string(), http_addr, osc_addr);
    instance.start_http_json().ok();

    loop {
        //instance.register_mdns_service();
        //instance.unregister_mdns_service();
        instance.attempt_force_vrc_response_detect(10).ok();
        std::thread::sleep(Duration::from_secs(10));
    }
}

#[test]
#[ignore]
fn detect_vrchat() {
    let osc_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9069);
    let http_addr = SocketAddrV4::new(Ipv4Addr::new(172, 19, 19, 244), 8080);

    let mut instance = OSCQuery::new("VibeCheck".to_string(), http_addr, osc_addr);
    let resolved_vrc_service = instance
        .mdns_search("VRChat-Client-".to_string(), OSC_JSON_SERVICE)
        .ok()
        .unwrap();
    info!(
        "[+] Got VRChat OSC JSON: {}",
        resolved_vrc_service.get_hostname()
    );
}
