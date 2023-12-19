use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct OSCQueryNode {
    #[serde(rename = "FULL_PATH")]
    pub full_path: String,
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
    pub fn node_at_path(&self, path: String) -> Option<&OSCQueryNode> {
        if self.full_path == path {
            return Some(self);
        }

        let NodeData::Internal(node) = &self.data else {
            return None;
        };

        let suffix = if self.full_path == "/" {
            path.as_str()
        } else {
            path.strip_prefix(&self.full_path)?
        };

        let next_parameter = suffix
            .chars()
            .skip(1)
            .take_while(|&c| c != '/')
            .collect::<String>();

        let child = node.contents.get(&next_parameter)?;
        return child.node_at_path(path);
    }

    pub fn leaf_params(&self) -> Vec<&str> {
        return match &self.data {
            NodeData::Leaf(_) => vec![&self.full_path],
            NodeData::Internal(internal) => {
                return internal
                    .contents
                    .iter()
                    .flat_map(|(_, node)| node.leaf_params())
                    .collect()
            }
        };
    }
}
