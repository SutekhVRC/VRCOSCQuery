//! mDNS service types and names used by OSCQuery.

/// OSCQuery's HTTP (JSON) service. VRChat finds OSCQuery apps by this type.
pub const OSC_JSON_SERVICE: &str = "_oscjson._tcp.local.";

/// A plain OSC (UDP) service.
pub const OSC_SERVICE: &str = "_osc._udp.local.";

/// VRChat's services are named `VRChat-Client-<random>`.
pub const VRCHAT_PREFIX: &str = "VRChat-Client-";

/// An mDNS-safe version of `name`: letters, digits and `-` only, so it can
/// be used in an instance and host name.
pub(crate) fn dns_label(name: &str) -> String {
    let label: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let label = label.trim_matches('-');
    if label.is_empty() {
        "OSCQuery".to_owned()
    } else {
        label.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_label_keeps_only_safe_characters() {
        assert_eq!(dns_label("VibeCheck"), "VibeCheck");
        assert_eq!(dns_label("My App 2!"), "My-App-2");
        assert_eq!(dns_label("  "), "OSCQuery");
    }
}
