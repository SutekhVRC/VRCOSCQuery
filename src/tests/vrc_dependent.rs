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
/// `HOST_INFO` gives 127.0.0.1, but VRChat used to ignore `OSC_IP` and send
/// to the machine's LAN address instead (see VRChatOSCQueryBugs.md). So the
/// same port is also bound on the LAN address, and the test reports which
/// address VRChat sent to. Either way nothing is sent to a closed port.
///
/// Both sockets stay bound until VRChat has been quiet for 5 seconds (up to
/// 60 s after the goodbye).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs VRChat"]
async fn vrchat_sends_to_an_advertised_port() {
    // Shows HTTP requests (did VRChat ask for HOST_INFO, and on which
    // address?) and, with RUST_LOG=vrcoscquery=debug,mdns_sd=debug, the
    // mDNS traffic
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("vrcoscquery=debug"),
    )
    .is_test(true)
    .try_init();

    let loopback = UdpSocket::bind("127.0.0.1:0").unwrap();
    let SocketAddr::V4(osc_addr) = loopback.local_addr().unwrap() else {
        unreachable!()
    };
    let lan_ip = lan_ip();
    let lan = UdpSocket::bind((lan_ip, osc_addr.port())).unwrap();
    for socket in [&loopback, &lan] {
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
    }

    let service = OscQueryService::start("VRCOSCQuery Test", osc_addr)
        .await
        .unwrap();
    println!(
        "Advertising OSC on {osc_addr} (also listening on {lan_ip}:{}), HTTP on port {}; waiting up to 30 s for VRChat",
        osc_addr.port(),
        service.http_addr().port()
    );

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut counts = Counts::default();
    let started = Instant::now();
    while Instant::now() < deadline && counts.total() < 50 {
        counts.poll(&loopback, &lan, started);
    }
    println!(
        "Received {} OSC packets on 127.0.0.1 and {} on {lan_ip}; first after {:?}",
        counts.loopback, counts.lan, counts.first_at
    );

    service.stop();
    let goodbye = Instant::now();
    let mut after = Counts::default();
    let mut last_total = 0;
    let mut last_packet = Instant::now();
    while last_packet.elapsed() < Duration::from_secs(5)
        && goodbye.elapsed() < Duration::from_secs(60)
    {
        after.poll(&loopback, &lan, goodbye);
        if after.total() > last_total {
            last_total = after.total();
            last_packet = Instant::now();
        }
    }
    println!(
        "After the goodbye: {} packets on 127.0.0.1 and {} on {lan_ip}, the last {:?} after it{}",
        after.loopback,
        after.lan,
        (after.total() > 0).then(|| last_packet - goodbye),
        if goodbye.elapsed() >= Duration::from_secs(60) {
            " (VRChat was still sending after 60 s)"
        } else {
            ""
        }
    );
    drop(loopback);
    drop(lan);
    assert!(
        counts.total() > 0,
        "VRChat didn't send anything to the advertised port"
    );
}

#[derive(Default)]
struct Counts {
    loopback: usize,
    lan: usize,
    first_at: Option<Duration>,
}

impl Counts {
    fn total(&self) -> usize {
        self.loopback + self.lan
    }

    fn poll(&mut self, loopback: &UdpSocket, lan: &UdpSocket, since: Instant) {
        let mut buffer = [0u8; 1536];
        if loopback.recv(&mut buffer).is_ok() {
            self.loopback += 1;
            self.first_at.get_or_insert(since.elapsed());
        }
        if lan.recv(&mut buffer).is_ok() {
            self.lan += 1;
            self.first_at.get_or_insert(since.elapsed());
        }
    }
}

/// This machine's LAN address: the one it would use to reach the internet.
/// Connecting a UDP socket sends nothing.
fn lan_ip() -> std::net::Ipv4Addr {
    let probe = UdpSocket::bind("0.0.0.0:0").unwrap();
    probe.connect("8.8.8.8:80").unwrap();
    match probe.local_addr().unwrap() {
        SocketAddr::V4(addr) => *addr.ip(),
        SocketAddr::V6(_) => unreachable!(),
    }
}
