//! Tests that need VRChat running on this machine, with OSC enabled.
//! Run them with `cargo test -- --ignored --nocapture vrc_dependent`.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::discovery;
use crate::OscQueryService;

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs VRChat"]
async fn finds_vrchat_and_its_osc_port() {
    let vrchat = discovery::find_vrchat(TIMEOUT)
        .await
        .unwrap()
        .expect("VRChat's OSCQuery service wasn't found");
    let info = discovery::host_info(&vrchat, TIMEOUT).await.unwrap();
    println!(
        "Found {} at {}: receives OSC on {}:{}",
        vrchat.name, vrchat.addr, info.osc_ip, info.osc_port
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs VRChat"]
async fn reads_vrchats_avatar_parameters() {
    let vrchat = discovery::find_vrchat(TIMEOUT)
        .await
        .unwrap()
        .expect("VRChat's OSCQuery service wasn't found");
    let parameters = discovery::node(&vrchat, "/avatar/parameters", TIMEOUT)
        .await
        .unwrap();
    let leaves = parameters.leaves();
    println!("{} avatar parameters", leaves.len());
    for leaf in leaves.iter().take(10) {
        println!("  {} {:?}", leaf.full_path, leaf.value());
    }
    assert!(!leaves.is_empty());
}

/// Advertises a service and checks VRChat sends OSC to it. Move around or
/// change a parameter in VRChat while it runs.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs VRChat"]
async fn vrchat_sends_to_an_advertised_port() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let SocketAddr::V4(osc_addr) = socket.local_addr().unwrap() else {
        unreachable!()
    };
    let service = OscQueryService::start("VRCOSCQuery Test", osc_addr)
        .await
        .unwrap();
    println!(
        "Advertising OSC on {}; waiting up to 30 s for VRChat",
        osc_addr
    );

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut buffer = [0u8; 1536];
    let mut received = 0;
    while Instant::now() < deadline && received < 20 {
        if socket.recv(&mut buffer).is_ok() {
            received += 1;
        }
    }
    service.stop();
    println!("Received {} OSC packets", received);
    assert!(
        received > 0,
        "VRChat didn't send anything to the advertised port"
    );
}
