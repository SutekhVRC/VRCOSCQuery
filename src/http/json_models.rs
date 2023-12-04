use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct HostInfoExtensions {
    #[serde(rename = "ACCESS")]
    pub access: bool,
    #[serde(rename = "CLIPMODE")]
    pub clipmode: bool,
    #[serde(rename = "RANGE")]
    pub range: bool,
    #[serde(rename = "TYPE")]
    pub _type: bool,
    #[serde(rename = "VALUE")]
    pub value: bool,
}

#[derive(Serialize, Deserialize)]
pub struct HostInfo<'hostinfo> {
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "EXTENSIONS")]
    pub extensions: HostInfoExtensions,
    #[serde(rename = "OSC_IP")]
    pub osc_ip: String,
    #[serde(rename = "OSC_PORT")]
    pub osc_port: u16,
    #[serde(rename = "OSC_TRANSPORT")]
    pub osc_transport: &'hostinfo str,
}
