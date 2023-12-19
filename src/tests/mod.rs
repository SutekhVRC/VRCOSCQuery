use super::*;
use crate::node::OSCQueryNode;
use once_cell::sync::Lazy;
use std::{collections::HashSet, fs};

pub mod vrc_dependent;

static ROOT_NODE: Lazy<OSCQueryNode> =
    Lazy::<_>::new(|| parse_relative_filepath("./src/tests/example_OGB.json").unwrap());

fn parse_relative_filepath(filepath: &str) -> Result<OSCQueryNode, anyhow::Error> {
    let data = fs::read_to_string(filepath)?;
    let node = serde_json::from_str::<OSCQueryNode>(&data)?;
    Ok(node)
}

#[test]
fn known_path_exists() {
    let path_to_get = "/avatar/parameters/vibecheck";
    let params = ROOT_NODE.node_at_path(path_to_get.to_string());
    assert_eq!(params.unwrap().full_path, path_to_get)
}

#[test]
fn wrong_path_is_none() {
    let path_to_get = "/path/that/does/not/exist";
    let params = ROOT_NODE.node_at_path(path_to_get.to_string());
    assert!(params.is_none())
}

#[test]
fn empty_path_is_none() {
    let path_to_get = "";
    let params = ROOT_NODE.node_at_path(path_to_get.to_string());
    assert!(params.is_none())
}

#[test]
fn leaf_params_on_subnode() {
    let path_to_get = "/avatar/parameters/OGB/Pen";
    let ogb_pen_node = ROOT_NODE.node_at_path(path_to_get.to_string()).unwrap();
    let ogb_pen_params = HashSet::from_iter(ogb_pen_node.leaf_params());
    let expected = HashSet::from([
        "/avatar/parameters/OGB/Pen/Mesh/PenSelf",
        "/avatar/parameters/OGB/Pen/Mesh/PenOthers",
    ]);
    assert_eq!(ogb_pen_params, expected)
}
