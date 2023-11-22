# VRChat OSCQuery implementation bugs

## Menu race condition

When an app has registered (responding to mDNS queries) an mDNS service. Enabling and disabling OSC in the expressions menu causes a race condition which triggers a state bug. I have found a temporary fix: Switch on the OSC toggle then off and on again quickly. This will trigger the race condition again flipping the state bug back to the correct state.

## OSC/JSON OSC_IP bug

The VRChat client does not read/parse the OSC IP in an OSC app's TCP/JSON service. Instead of reading the OSC_IP from the TCP/JSON service the VRChat client uses the default LAN interfaces address.

## OSC/JSON Service Bind Address

The VRChat application will not listen to the mDNS response with the TCP/JSON's A record. Instead it will request the default LAN address.