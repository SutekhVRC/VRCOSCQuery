# VRChat Client's OSCQuery implementation bugs

These are bugs I find with the VRChat client's OSCQuery implementation.

## Menu race condition

**Bug:**
- When an app has registered (responding to mDNS queries) an mDNS service. Enabling and disabling OSC in the expressions menu causes a race condition which triggers a state bug.

**Workaround:**
- Switch on the OSC toggle then off and on again quickly. This will trigger the race condition again flipping the state bug back to the correct state.

## OSC/JSON OSC_IP bug

**Bug:**
- The VRChat client does not read/parse the OSC IP in an OSC app's TCP/JSON service. Instead of reading the OSC_IP from the TCP/JSON service the VRChat client uses the default LAN interfaces address.

**Still there (tested 2026-11-14, Linux/Proton):** `HOST_INFO` gave `127.0.0.1`, and all 50 packets VRChat sent went to the LAN address (`192.168.1.13`) on the advertised port, none to `127.0.0.1`. It does use `OSC_PORT`.

**Workaround:**
- Bind your app to your LAN interface :) (Security risk)

## OSC/JSON Service Bind Address

**Bug:**
- The VRChat application will not listen to the mDNS response with the TCP/JSON's A record. Instead it will send the HTTP request the default LAN address.

**Workaround:**
- Bind your TCP/JSON service to your LAN interface :) (Security risk)

**Not settled (2026-11-14):** VRChat connected to the HTTP server on the LAN address, but that was also the address in the A record, so this test can't tell whether it reads the A record. VRCOSCQuery's HTTP server listens on all interfaces, so it works either way.

## VRChat ignores mDNS requests without additional fields

**Bug:**
- The VRChat client will ignore certain mDNS requests that sent their records within the answers instead of using Additional Answers.

**Workaround:**
- Certain mDNS implementations do not expect this behavior so you may need to induce separation of answers into additional answers.
- Craft your own mDNS responses that always separate your PTR answer into the Answers section and the rest of the records into the Additional Answers.

**Confirmed (2026-11-14):** with mdns-sd, VRChat never found the service, because the only responses mdns-sd sends with that split are answers to PTR questions, and VRChat never asks a PTR question (see the next section). Its announcements put all four records (PTR, SRV, TXT, A) in Answers, which VRChat ignores. Answering VRChat's ANY questions the PTR way fixed it: VRChat asked for `/` and `/?HOST_INFO` within one query cycle and started sending OSC.

## VRChat browses with ANY questions

**Behaviour:**
- Every 10 seconds VRChat sends two queries from port 5353: `_osc._udp.local.` and `_oscjson._tcp.local.`, both with record type **ANY** (255), not PTR, and without the unicast-response (QU) bit.
- It answers its own queries with its own services (`VRChat-Client-XXXXXX`), PTR in Answers and SRV/TXT/A in Additionals. Its `_oscjson._tcp` A record is `127.0.0.1`; its `_osc._udp` A record is the LAN address.

**Bug (in mdns-sd, not VRChat):**
- mdns-sd (0.9.3 and 0.21.7) only lists its services for PTR questions. An ANY question for a service type matches no host or instance name, so it's ignored and VRChat never hears about the service.

**Fix:**
- Answer an ANY question for one of your service types like a PTR question (PTR in Answers, SRV/TXT/A in Additionals). It's a 13-line change in mdns-sd's `handle_query`; an ANY question asks for every record of that name, which for a service type is its PTRs, so it's correct DNS too.

## VRChat ignores mDNS goodbyes

**Bug:**
- When a service stops advertising (mDNS goodbye: its records sent again with TTL 0), VRChat keeps sending OSC to it. Tested 2026-11-14: 1165 packets over more than 60 seconds after the goodbye, still going. It probably keeps the service until its records expire (mdns-sd's PTR TTL is 4500 s) or VRChat restarts.

**Workaround:**
- Keep the OSC port bound for as long as the app runs, and reuse the same port next time (save it), so VRChat never sends to a closed port. Failed sends to a closed port count toward VRChat's "OSC sending failed too often", after which it stops sending OSC until restarted (seen on Linux/Proton).
- In that test the port was closed while VRChat was still sending, and VRChat's OSC did not die (another app was listening on 9001 throughout), so the limit may only count sends to some ports. Still, don't rely on it.

## VRChat keeps sending to 9001

**Behaviour:**
- VRChat keeps sending to its default `127.0.0.1:9001` as well as to every OSCQuery service it finds. Finding a service doesn't replace the default.

**Bug (Linux/Proton, tested 2026-11-14):**
- With nothing listening on 9001, VRChat's sends there fail, and VRChat logs "OSC sending failed too often disabling for this session" and stops sending OSC entirely (to the OSCQuery service too) until it restarts. With an app moved to its own OSCQuery port, nothing may be on 9001 any more.

**Workaround:**
- While advertising, also hold `127.0.0.1:9001` if it's free: bind it and don't read it. The OS drops the data once the socket's buffer is full, and VRChat's sends succeed. The same data arrives on the advertised port. If another app has 9001, that app is listening; try again now and then in case it lets go.
- Also keep any gap in listening short: an app that unbinds its OSC port for even a second while VRChat sends (e.g. when switching listeners) can hit the limit.