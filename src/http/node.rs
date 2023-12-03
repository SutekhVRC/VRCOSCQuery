use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OSCQueryValue {
    BOOLEAN(bool),
    FLOAT(f32),
    STRING(String),
    INT(i32),
    // Havent seen any non-empty response from VRChat for "VALUE":[{}]
    OBJECT(HashMap<String, u8>),
}

#[derive(Serialize, Deserialize)]
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

impl std::fmt::Debug for OSCQueryNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pretty_string = serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", pretty_string)
    }
}

impl OSCQueryNode {
    pub fn get_parameter(&self) -> Option<Vec<String>> {
        let root_node = self.clone();

        let avatar_params = root_node.contents.as_ref().unwrap().get("avatar").unwrap();

        None
    }
}