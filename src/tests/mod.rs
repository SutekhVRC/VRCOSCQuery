use std::net::Ipv4Addr;

use super::*;

#[test]
fn instantiate() {
    let instance = OSCQuery::new(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));
}