
use std::collections::HashMap;

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
    pub value: bool
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OSCQueryValue {
    BOOLEAN(bool),
    FLOAT(f32),
    STRING(String),
    INT(i32),
    // Havent seen any non-empty response from VRChat for "VALUE":[{}]
    OBJECT(HashMap<String, u8>)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OSCQueryNode {
    #[serde(skip_serializing_if = "Option::is_none", rename = "DESCRIPTION")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "FULL_PATH")]
    full_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ACCESS")]
    access: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "TYPE")]
    _type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "VALUE")]
    value: Option<Vec<OSCQueryValue>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "CONTENTS")]
    contents: Option<HashMap<String, Self>>,
}

impl OSCQueryNode {
    pub fn get_avatar_parameters(&self) -> Option<Vec<String>> {
        None
    }

}