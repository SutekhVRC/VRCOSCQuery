# VRCOSCQuery

OSCQuery for VRChat apps, in Rust.

- **Advertise your app** (`OscQueryService`): VRChat finds it over mDNS and sends it OSC on the port you chose, alongside its usual 9001. No fixed port to clash with other OSC apps.
- **Find and read VRChat** (`discovery`): find VRChat's OSCQuery service with a timeout, read its `HOST_INFO` (e.g. the port VRChat receives OSC on) and its node tree (every avatar parameter with its current value).

Everything runs on your tokio runtime, and nothing blocks forever: discovery takes a timeout and returns `None` if nothing turns up.

## Advertise your app

```rust
use std::net::SocketAddr;

// Bind your OSC socket on a free port, on all interfaces: VRChat sends to
// the LAN address whatever HOST_INFO says (see below)
let osc = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
let SocketAddr::V4(osc_addr) = osc.local_addr()? else { unreachable!() };

// VRChat will now send avatar data to `osc_addr`
let service = vrcoscquery::OscQueryService::start("MyApp", osc_addr).await?;

// ... receive OSC on `osc` ...

// Stop advertising (dropping the service does the same)
service.stop();
```

The service:
- registers `_oscjson._tcp` (its HTTP server) and `_osc._udp` (your OSC port) on this machine's network interfaces;
- serves `/?HOST_INFO` (your OSC address) and a node tree with `/avatar/change`, which is what tells VRChat to send avatar data;
- on stop or drop, sends mDNS goodbyes and frees its HTTP port.

mDNS is done with [mdns-sd](https://crates.io/crates/mdns-sd), from our fork ([SutekhVRC/mdns-sd](https://github.com/SutekhVRC/mdns-sd/tree/answer-any-for-service-type), pinned to a commit) with one fix VRChat needs: VRChat browses with ANY questions (not PTR), and only accepts answers with the PTR in Answers and SRV/TXT/A in Additionals. mdns-sd only sends that for PTR questions, so the fix answers ANY questions for a service type the same way. Without it VRChat never finds the service. See [VRChatOSCQueryBugs.md](VRChatOSCQueryBugs.md).

If VRChat started after your service and hasn't picked it up, `service.reannounce()` announces it again.

Things VRChat does that your app has to allow for (details in [VRChatOSCQueryBugs.md](VRChatOSCQueryBugs.md)):
- **It ignores `OSC_IP`** and sends to this machine's LAN address on `OSC_PORT`. Bind your OSC socket to `0.0.0.0` (or the LAN address); a socket on `127.0.0.1` alone gets nothing. This opens the port to your local network.
- **It keeps sending to 9001** as well, whatever it finds. On Linux/Proton, if nothing listens there, VRChat stops sending OSC altogether. Hold `127.0.0.1:9001` while it's free (bind it, don't read it).
- **It ignores the goodbye** and keeps sending to a service it found, probably until VRChat restarts. Keep your OSC socket bound for as long as your app runs, and reuse the same port next time. Failed sends to a closed port can count toward VRChat's "OSC sending failed too often" (seen through Proton), after which it stops sending OSC until restarted. Never advertise a port nothing listens on.

## Find and read VRChat

```rust
use std::time::Duration;
use vrcoscquery::discovery;

let timeout = Duration::from_secs(3);
if let Some(vrchat) = discovery::find_vrchat(timeout).await? {
    let info = discovery::host_info(&vrchat, timeout).await?;
    println!("VRChat receives OSC on {}:{}", info.osc_ip, info.osc_port);

    let parameters = discovery::node(&vrchat, "/avatar/parameters", timeout).await?;
    for leaf in parameters.leaves() {
        println!("{} = {:?}", leaf.full_path, leaf.value());
    }
}
```

`discovery::find_service(service_type, name_prefix, timeout)` finds any other OSCQuery or OSC service the same way.

## Tests

```bash
cargo test                                    # nothing is advertised on the network
cargo test -- --ignored finds_its_own         # mDNS round trip (needs multicast; avoid while VRChat runs)
cargo test -- --ignored --nocapture vrc_dependent   # needs VRChat running with OSC on
```

`vrc_dependent::vrchat_sends_to_an_advertised_port` advertises a service and waits up to 30 seconds for VRChat to send it OSC (move around or change a parameter while it runs). It then stops advertising and reports how long VRChat kept sending, keeping the socket bound until VRChat has been quiet for 5 seconds.
