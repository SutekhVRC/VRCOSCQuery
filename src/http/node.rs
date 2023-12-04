use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct OSCQueryNode {
    #[serde(rename = "FULL_PATH")]
    full_path: String,
    #[serde(rename = "ACCESS")]
    access: i8,
    #[serde(skip_serializing_if = "Option::is_none", rename = "DESCRIPTION")]
    description: Option<String>,
    #[serde(flatten)]
    data: NodeData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeData {
    Internal(Contents),
    Leaf(TypedValue),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Contents {
    #[serde(rename = "CONTENTS")]
    contents: HashMap<String, OSCQueryNode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedValue {
    #[serde(rename = "TYPE")]
    _type: String,
    #[serde(rename = "VALUE")]
    value: Option<Vec<OSCQueryValue>>,
}

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

impl std::fmt::Debug for OSCQueryNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pretty_string = serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", pretty_string)
    }
}

impl OSCQueryNode {
    pub fn get_full_path(&self) -> &str {
        return &self.full_path;
    }

    pub fn node_at_path(&self, path: String) -> Option<&OSCQueryNode> {
        if self.full_path == path {
            return Some(self);
        }

        match &self.data {
            NodeData::Leaf(_node) => return None,
            NodeData::Internal(node) => {
                // Bad suffix hack because when we slice on root, we remove first /
                //  But when slicing on other nodes, we don't remove leading /
                let suffix;
                if self.full_path == "/" {
                    suffix = Some(path.as_str())
                } else {
                    suffix = path.strip_prefix(&self.full_path)
                }
                if let Some(remaining_path) = suffix {
                    let attr = remaining_path[1..]
                        .chars()
                        .take_while(|&c| c != '/')
                        .collect::<String>();
                    match node.contents.get(&attr) {
                        Some(child) => {
                            return child.node_at_path(path.to_string());
                        }
                        None => return None,
                    }
                } else {
                    return None;
                }
            }
        }
    }
}
