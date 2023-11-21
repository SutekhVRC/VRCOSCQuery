use std::net::Ipv4Addr;

use super::*;

#[test_log::test]
fn instantiate_http() {
    let _ = env_logger::builder().is_test(true).try_init();
    let mut instance = OSCQuery::new(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));
    instance.start_oq();
    std::thread::sleep(Duration::from_secs(10));
    info!("Sending stop signal!");
    instance.stop_oq();
}