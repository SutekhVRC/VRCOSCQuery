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

**Workaround:**
- Bind your app to your LAN interface :) (Security risk)

## OSC/JSON Service Bind Address

**Bug:**
- The VRChat application will not listen to the mDNS response with the TCP/JSON's A record. Instead it will send the HTTP request the default LAN address.

**Workaround:**
- Bind your TCP/JSON service to your LAN interface :) (Security risk)

## VRChat ignores mDNS requests without additional fields

**Bug:**
- The VRChat client will ignore certain mDNS requests that sent their records within the answers instead of using Additional Answers.

**Workaround:**
- Certain mDNS implementations do not expect this behavior so you may need to induce separation of answers into additional answers.
- Craft your own mDNS responses