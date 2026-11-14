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

/// Advertises a service and checks VRChat sends OSC to it, then stops
/// advertising and measures how long VRChat keeps sending to the port.
/// Move around or change a parameter in VRChat while it runs.
///
/// The socket stays bound until VRChat has been quiet for 5 seconds (up to
/// 60 s after the goodbye), so VRChat never sends to a closed port.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs VRChat"]
async fn vrchat_sends_to_an_advertised_port() {
    // Shows HTTP requests (did VRChat ask for HOST_INFO?) and, with
    // RUST_LOG=vrcoscquery=debug,mdns_sd=debug, the mDNS traffic
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("vrcoscquery=debug"),
    )
    .is_test(true)
    .try_init();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let SocketAddr::V4(osc_addr) = socket.local_addr().unwrap() else {
        unreachable!()
    };
    let service = OscQueryService::start("VRCOSCQuery Test", osc_addr)
        .await
        .unwrap();
    println!(
        "Advertising OSC on {osc_addr} (HTTP on port {}); waiting up to 30 s for VRChat",
        service.http_addr().port()
    );

    let mut buffer = [0u8; 1536];
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut received = 0;
    let mut first_at = None;
    let started = Instant::now();
    while Instant::now() < deadline && received < 50 {
        if socket.recv(&mut buffer).is_ok() {
            received += 1;
            first_at.get_or_insert(started.elapsed());
        }
    }
    println!("Received {received} OSC packets; first after {first_at:?}");

    service.stop();
    let goodbye = Instant::now();
    let mut after_goodbye = 0;
    let mut last_at = None;
    let mut last_packet = Instant::now();
    while last_packet.elapsed() < Duration::from_secs(5)
        && goodbye.elapsed() < Duration::from_secs(60)
    {
        if socket.recv(&mut buffer).is_ok() {
            after_goodbye += 1;
            last_packet = Instant::now();
            last_at = Some(goodbye.elapsed());
        }
    }
    println!(
        "After the goodbye: {after_goodbye} packets, the last {last_at:?} after it{}",
        if goodbye.elapsed() >= Duration::from_secs(60) {
            " (VRChat was still sending after 60 s)"
        } else {
            ""
        }
    );
    drop(socket);
    assert!(
        received > 0,
        "VRChat didn't send anything to the advertised port"
    );
}
