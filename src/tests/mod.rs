use once_cell::sync::Lazy;
use std::fs;

use super::*;

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
