//! An app's service end to end, over real sockets.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

use crate::discovery::{self, Endpoint};
use crate::mdns::OSC_JSON_SERVICE;
use crate::OscQueryService;

const TIMEOUT: Duration = Duration::from_secs(3);

fn free_osc_addr() -> (UdpSocket, SocketAddrV4) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let SocketAddr::V4(addr) = socket.local_addr().unwrap() else {
        unreachable!()
    };
    (socket, addr)
}

fn http_endpoint(service: &OscQueryService) -> Endpoint {
    Endpoint {
        name: "test".into(),
        addr: SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::LOCALHOST,
            service.http_addr().port(),
        )),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn serves_host_info_and_the_avatar_tree() {
    let (_socket, osc_addr) = free_osc_addr();
    let service = OscQueryService::start("VRCOSCQuery Test", osc_addr)
        .await
        .unwrap();
    let endpoint = http_endpoint(&service);

    let info = discovery::host_info(&endpoint, TIMEOUT).await.unwrap();
    assert_eq!(info.name, "VRCOSCQuery Test");
    assert_eq!(info.osc_ip, "127.0.0.1");
    assert_eq!(info.osc_port, osc_addr.port());

    let root = discovery::node(&endpoint, "/", TIMEOUT).await.unwrap();
    let change = root
        .node_at_path("/avatar/change".to_string())
        .expect("VRChat needs /avatar/change to send avatar data");
    assert_eq!(change.type_tag(), Some("s"));

    let avatar = discovery::node(&endpoint, "/avatar", TIMEOUT)
        .await
        .unwrap();
    assert_eq!(avatar.full_path, "/avatar");

    assert!(discovery::node(&endpoint, "/nope", TIMEOUT).await.is_err());
    service.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn advertises_localhost_for_an_unspecified_address() {
    let service = OscQueryService::start("VRCOSCQuery Test", "0.0.0.0:9555".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(service.osc_addr(), "127.0.0.1:9555".parse().unwrap());
    let info = discovery::host_info(&http_endpoint(&service), TIMEOUT)
        .await
        .unwrap();
    assert_eq!(info.osc_ip, "127.0.0.1");
}

#[tokio::test(flavor = "multi_thread")]
async fn stopping_frees_the_http_port() {
    let (_socket, osc_addr) = free_osc_addr();
    let service = OscQueryService::start("VRCOSCQuery Test", osc_addr)
        .await
        .unwrap();
    let http_addr = service.http_addr();
    service.stop();

    // The listener is gone, so the port can be bound again
    tokio::time::sleep(Duration::from_millis(50)).await;
    std::net::TcpListener::bind(http_addr).expect("port still in use");
}

#[tokio::test(flavor = "multi_thread")]
async fn discovery_gives_up_quietly_after_the_timeout() {
    let start = Instant::now();
    let found = discovery::find_service(
        OSC_JSON_SERVICE,
        "NoSuchService-vrcoscquery-test",
        Duration::from_millis(500),
    )
    .await
    .unwrap();
    assert!(found.is_none());
    assert!(start.elapsed() < Duration::from_secs(3));
}

/// Needs mDNS multicast to work on this machine (it usually does on a
/// desktop; CI runners and sandboxes may block it).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs mDNS multicast"]
async fn finds_its_own_service_over_mdns() {
    let (_socket, osc_addr) = free_osc_addr();
    let name = format!("VRCOSCQueryFind{}", std::process::id());
    let service = OscQueryService::start(&name, osc_addr).await.unwrap();

    let found = discovery::find_service(OSC_JSON_SERVICE, &name, Duration::from_secs(5))
        .await
        .unwrap()
        .expect("service not found over mDNS");
    assert_eq!(found.addr.port(), service.http_addr().port());

    let info = discovery::host_info(&found, TIMEOUT).await.unwrap();
    assert_eq!(info.osc_port, osc_addr.port());
}
