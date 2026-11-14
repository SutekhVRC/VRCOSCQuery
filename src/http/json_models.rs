use serde::{Deserialize, Serialize};

/// Which OSCQuery extensions a service supports. Missing ones read as
/// unsupported.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct HostInfoExtensions {
    #[serde(rename = "ACCESS", default)]
    pub access: bool,
    #[serde(rename = "CLIPMODE", default)]
    pub clipmode: bool,
    #[serde(rename = "RANGE", default)]
    pub range: bool,
    #[serde(rename = "TYPE", default)]
    pub _type: bool,
    #[serde(rename = "VALUE", default)]
    pub value: bool,
}

/// A service's `/?HOST_INFO`: its name, and where it receives OSC.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct HostInfo {
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "EXTENSIONS", default)]
    pub extensions: HostInfoExtensions,
    #[serde(rename = "OSC_IP")]
    pub osc_ip: String,
    #[serde(rename = "OSC_PORT")]
    pub osc_port: u16,
    #[serde(rename = "OSC_TRANSPORT", default = "udp")]
    pub osc_transport: String,
}

fn udp() -> String {
    "UDP".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_vrchats_host_info_and_ignores_unknown_fields() {
        // Shaped like VRChat's reply
        let info: HostInfo = serde_json::from_str(
            r#"{"NAME":"VRChat-Client-ABC123","EXTENSIONS":{"ACCESS":true,"CLIPMODE":false,
                "RANGE":true,"TYPE":true,"VALUE":true},"OSC_IP":"127.0.0.1","OSC_PORT":9000,
                "OSC_TRANSPORT":"UDP","WS_IP":"127.0.0.1","WS_PORT":0}"#,
        )
        .unwrap();
        assert_eq!(info.name, "VRChat-Client-ABC123");
        assert_eq!(info.osc_port, 9000);
        assert!(info.extensions.value);
    }
}
