use crate::http::node;
use std::{fs::File, io::Read};

use super::*;

pub mod vrc_dependent;

fn parse_rel_file(filepath: &str) -> Result<node::OSCQueryNode, serde_json::Error> {
    let mut file = File::open(filepath).unwrap();
    let mut json_data = String::new();
    file.read_to_string(&mut json_data).unwrap();
    return serde_json::from_str::<OSCQueryNode>(&json_data);
}

#[test]
fn test_path() {
    let root = parse_rel_file("./src/tests/example_OGB.json").unwrap();
    let path_to_get = "/avatar/parameters/vibecheck";
    let params = root.node_at_path(path_to_get.to_string()).unwrap();
    assert_eq!(params.get_full_path(), path_to_get)
}
